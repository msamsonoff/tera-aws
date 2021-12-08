mod ec2;
mod imds;
mod secretsmanager;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use aws_config::imds::region::ImdsRegionProvider;
use aws_config::meta::region::RegionProviderChain;
use aws_types::region::Region;
use eyre::Result;
use structopt::StructOpt;
use tempfile::NamedTempFile;
use tera::{Context, Tera};
use walkdir::WalkDir;

#[derive(StructOpt)]
struct Opt {
    /// Overrides any region value set by environment variable or configuration file
    #[structopt(name = "REGION", short = "r", long = "region")]
    region: Option<String>,

    /// The directory containing the Tera templates
    #[structopt(
        name = "TEMPLATE-DIR",
        short = "d",
        long = "template-dir",
        default_value = "./templates"
    )]
    template_dir: PathBuf,

    /// The path of the root template
    #[structopt(name = "TEMPLATE")]
    template_name: PathBuf,

    /// The path to the output file
    #[structopt(name = "OUTPUT")]
    output_path: PathBuf,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let Opt {
        region,
        template_dir,
        template_name,
        output_path,
    } = Opt::from_args();

    let template_dir = template_dir
        .canonicalize()
        .map_err_with_path(template_dir)?;
    let mut files = Vec::new();
    for dir_entry in WalkDir::new(&template_dir) {
        let dir_entry = dir_entry?;
        let metadata = dir_entry.metadata()?;
        if metadata.is_file() {
            let path = dir_entry.path();
            let path = path.canonicalize().map_err_with_path(path)?;
            let name = path
                .to_str()
                .ok_or_else(|| TeraAwsError::InvalidUtf8Path(path.clone()))?
                .to_string();
            files.push((path, Some(name)));
        }
    }
    let mut tera = Tera::default();
    tera.add_template_files(files)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let runtime = Arc::new(runtime);
    let future = async {
        let region_provider = RegionProviderChain::first_try(region.map(Region::new))
            .or_default_provider()
            .or_else(ImdsRegionProvider::builder().build());
        aws_config::from_env().region(region_provider).load().await
    };
    let config = runtime.block_on(future);
    crate::ec2::register(&mut tera, &runtime, &config);
    crate::imds::register(&mut tera, &runtime)?;
    crate::secretsmanager::register(&mut tera, &runtime, &config);

    let template_name = template_name
        .canonicalize()
        .map_err_with_path(template_name)?;
    let template_name = template_name
        .to_str()
        .ok_or_else(|| TeraAwsError::InvalidUtf8Path(template_name.clone()))?;
    let context = Context::new();
    let file = if let Some(dir) = output_path.parent() {
        NamedTempFile::new_in(dir)
    } else {
        NamedTempFile::new()
    }?;
    tera.render_to(template_name, &context, &file)?;
    file.persist(output_path)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum TeraAwsError {
    #[error("{0}: {1}")]
    IoError(PathBuf, std::io::Error),

    #[error("{0}: Path is not valid UTF-8")]
    InvalidUtf8Path(PathBuf),
}

trait ResultExt<T> {
    fn map_err_with_path<P>(self, path: P) -> Result<T, TeraAwsError>
    where
        P: AsRef<Path>;
}

impl<T> ResultExt<T> for std::io::Result<T> {
    fn map_err_with_path<P>(self, path: P) -> Result<T, TeraAwsError>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref().to_path_buf();
        self.map_err(|err| TeraAwsError::IoError(path, err))
    }
}
