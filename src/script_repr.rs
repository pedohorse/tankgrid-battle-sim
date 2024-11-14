pub trait ToScriptRepr {
    fn to_script_repr(&self) -> String;
}

pub trait FromScriptRepr
where
    Self: Sized,
{
    fn from_script_repr(from: &str) -> Option<Self>;
}
