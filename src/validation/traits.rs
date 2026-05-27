pub trait Validate<'ctx> {
    type Context: 'ctx;
    type Error;
    fn validate(&self, ctx: &'ctx Self::Context) -> Result<(), Self::Error>;
}
