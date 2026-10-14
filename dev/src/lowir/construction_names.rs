// Translated from lowir/local_names.cpp. See NOTICE.
//! Constructed slot identities acquire presentation names after all source names are known.
use super::{
    Error,
    model::{NameId, Names},
};
use std::collections::{HashMap, HashSet};
pub(super) enum SlotName {
    Source(NameId),
    Generated(&'static str),
    Parameter(usize),
}
pub(super) fn slots(
    names: &mut Names,
    labels: &[SlotName],
    parameters: &[NameId],
) -> Result<Vec<NameId>, Error> {
    let mut taken: HashSet<NameId> = parameters.iter().copied().collect();
    taken.extend(labels.iter().filter_map(|label| match label {
        SlotName::Source(name) => Some(*name),
        _ => None,
    }));
    let mut counts = HashMap::<NameId, u32>::new();
    let mut generated = 0_u32;
    let mut result = Vec::with_capacity(labels.len());
    for label in labels {
        let text = match label {
            SlotName::Source(name) => {
                let count = counts.entry(*name).or_default();
                *count = count.checked_add(1).ok_or(Error::Limit)?;
                if *count == 1 {
                    result.push(*name);
                    continue;
                }
                format!("{}__shadow{}", names.get(*name), count)
            }
            SlotName::Generated(stem) => loop {
                generated = generated.checked_add(1).ok_or(Error::Limit)?;
                let text = format!("{stem}__{generated}");
                if !taken.contains(&names.intern(&text)?) {
                    break text;
                }
            },
            SlotName::Parameter(index) => format!("__param{index}"),
        };
        let mut text = text;
        while taken.contains(&names.intern(&text)?) {
            text.push('_');
        }
        result.push(names.intern(&text)?);
    }
    Ok(result)
}
