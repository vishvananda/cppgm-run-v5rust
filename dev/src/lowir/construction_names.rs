// Translated from lowir/local_names.cpp. See NOTICE.
//! Constructed slot identities acquire presentation names after all source names are known.
use super::{
    Error,
    model::{NameId, Names},
};
use std::collections::{HashMap, HashSet};
#[derive(Clone, Copy)]
pub(super) enum SlotName {
    Source(NameId),
    Shadow(NameId),
    Anonymous,
    RangeFor(NameId),
    Generated(&'static str),
    GeneratedName(NameId),
    Parameter(usize),
}
pub(super) fn record_shadows(labels: &mut [SlotName]) {
    let mut seen = HashSet::new();
    for label in labels {
        if let SlotName::Source(name) = *label
            && !seen.insert(name)
        {
            *label = SlotName::Shadow(name);
        }
    }
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
    let mut range_for = 0_u32;
    let mut result = Vec::with_capacity(labels.len());
    for (index, label) in labels.iter().enumerate() {
        let text = match label {
            SlotName::Source(name) | SlotName::Shadow(name) => {
                let count = counts.entry(*name).or_default();
                *count = count.checked_add(1).ok_or(Error::Limit)?;
                if matches!(label, SlotName::Source(_)) {
                    result.push(*name);
                    continue;
                }
                format!("{}__shadow{}", names.get(*name), count)
            }
            SlotName::Anonymous => index.to_string(),
            SlotName::Generated(stem) => loop {
                generated = generated.checked_add(1).ok_or(Error::Limit)?;
                let text = format!("{stem}__{generated}");
                if !taken.contains(&names.intern_view(&text)?) {
                    break text;
                }
            },
            SlotName::GeneratedName(stem) => loop {
                generated = generated.checked_add(1).ok_or(Error::Limit)?;
                let text = format!("{}__{generated}", names.get(*stem));
                if !taken.contains(&names.intern_view(&text)?) {
                    break text;
                }
            },
            SlotName::Parameter(index) => format!("__param{index}"),
            SlotName::RangeFor(name) => {
                range_for = range_for.checked_add(1).ok_or(Error::Limit)?;
                format!("{}{range_for}", names.get(*name))
            }
        };
        let mut text = text;
        while taken.contains(&names.intern_view(&text)?) {
            text.push('_');
        }
        result.push(names.intern_view(&text)?);
    }
    Ok(result)
}
