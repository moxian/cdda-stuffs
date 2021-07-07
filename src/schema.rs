type Map<K, V> = std::collections::HashMap<K, V>;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub(crate) enum Name {
    Plain(String),
    Same {
        str_sp: String,
    },
    WithPlural {
        str: String,
        #[allow(dead_code)]
        str_pl: Option<String>,
    },
}
impl Name {
    pub(crate) fn to_string(self) -> String {
        match self {
            Name::Plain(s) => s,
            Name::Same { str_sp } => str_sp,
            Name::WithPlural { str, .. } => str,
        }
    }
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Name::Plain(s) => s,
            Name::Same { str_sp } => str_sp,
            Name::WithPlural { str, .. } => str,
        }
    }
}
impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub(crate) struct Comestible {
    pub id: String,
    pub name: Name,
    pub description: String,
    // #[serde(default)]  // easier than Option<String>, and doesn't really matter.
    // pub category: String,
    #[serde(default)] // lye
    pub comestible_type: String, // enum
    #[serde(default)]
    pub quench: i32,
    #[serde(default)]
    pub healthy: i32,
    #[serde(default)]
    pub calories: i32,
    #[serde(default)]
    pub fun: i32,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(from = "ComponentDescRaw")]
pub(crate) enum ComponentDesc {
    Plain(String, i32),
    List(String, i32),
    // ListRaw(String, i32, String),
}
impl ComponentDesc {
    pub fn name(&self) -> &str {
        match self {
            Self::Plain(s, _) => s,
            Self::List(s, _) => s,
        }
    }
    pub fn amount(&self) -> i32 {
        match self {
            Self::Plain(_, i) => *i,
            Self::List(_, i) => *i,
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
enum ComponentDescRaw {
    Two(String, i32),
    Three(String, i32, String),
    ThreeList(String, i32, Vec<String>),
}
impl From<ComponentDescRaw> for ComponentDesc {
    fn from(f: ComponentDescRaw) -> ComponentDesc {
        use ComponentDescRaw::*;
        match f {
            Two(s, i) => ComponentDesc::Plain(s, i),
            Three(s, i, which) if which == "LIST" => ComponentDesc::List(s, i),
            Three(s, i, which) if which == "NO_RECOVER" => {
                // TODO: NO_RECOVER is more special than this
                ComponentDesc::List(s, i)
            }
            _ => unimplemented!("can't recognize {:?}", f),
        }
    }
}
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum RecipeTime {
    Human(String),
    TicksMaybe(i32),
}
impl Default for RecipeTime {
    fn default() -> Self {
        Self::TicksMaybe(0)
    }
}
impl RecipeTime {
    pub fn to_human(&self) -> String {
        match self {
            RecipeTime::Human(h) => h.to_string(),
            RecipeTime::TicksMaybe(t) => format!(" ??? {} ticks ???", t),
        }
    }
    pub fn to_seconds(&self) -> i32 {
        let s = self.to_human();
        let mut res = 0i32;
        let mut mult = 1;
        for c in s.chars().rev() {
            match c {
                's' => mult = 1,
                'm' => mult = 60,
                'h' => mult = 60 * 60,
                '0'..='9' => {
                    let digit = c.to_digit(10).unwrap() as i32;
                    res += digit * mult;
                    mult *= 10;
                }
                ' ' => {}
                _ => panic!("not a time thing {:?}", s),
            }
        }
        res
    }
}
#[derive(Clone, Debug, serde::Deserialize)]
pub(crate) struct Quality {
    pub id: String,
    pub level: i32,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum Autolearn {
    No,
    Yes,
    Complex(Vec<(String, i32)>),
}

impl Default for Autolearn {
    fn default() -> Self {
        Autolearn::No
    }
}

fn deserialize_autolearn<'de, D>(deserializer: D) -> Result<Autolearn, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum AL {
        Plain(bool),
        Complex(Vec<(String, i32)>),
    }
    use serde::Deserialize;
    let maybe_al = AL::deserialize(deserializer)?;
    Ok(match maybe_al {
        AL::Plain(false) => Autolearn::No,
        AL::Plain(true) => Autolearn::Yes,
        AL::Complex(v) => Autolearn::Complex(v),
    })
}

#[derive(Clone, Debug, serde::Deserialize)]
pub(crate) struct Recipe {
    pub result: String,
    pub id_suffix: Option<String>,
    #[serde(default)] //  see "saddlebag"
    pub skill_used: String, // enum?
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_vec_or_one")]
    pub skills_required: Vec<(String, i32)>,
    #[serde(default)]
    pub difficulty: i32,
    #[serde(default)] // see "seed_oats"
    pub time: RecipeTime,
    pub charges: Option<i32>,
    pub result_mult: Option<i32>, // default is 1, but eh
    pub components: Vec<Vec<ComponentDesc>>,
    #[serde(default)] // see "seed_oats"
    pub qualities: Vec<Quality>,
    #[serde(deserialize_with = "deserialize_autolearn", default)]
    pub autolearn: Autolearn,
    #[serde(default)]
    pub reversible: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub(crate) struct Material {
    #[serde(rename = "type")]
    pub typ: String,

    pub id: String,
    pub name: String,
    #[serde(default)] // blood and friends
    pub bash_resist: i32,
    #[serde(default)] // blood and friends
    pub cut_resist: i32,
    #[serde(default)] // blood and friends
    pub acid_resist: i32,
    #[serde(default)] // blood and friends
    pub elec_resist: i32,
    #[serde(default)] // blood and friends
    pub chip_resist: i32,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub(crate) struct Requirement {
    pub id: String,
    #[serde(rename = "//")]
    pub comment: Option<String>,
    #[serde(default)] // sometimes we only have tools. See "22_casehead"
    pub components: Vec<Vec<ComponentDesc>>,
}

#[derive(Clone, Debug, Default, serde::Serialize, Copy)]
pub(crate) struct Volume {
    // s: String,
    pub ml: i32,
}

impl<'de> serde::Deserialize<'de> for Volume {
    fn deserialize<D>(deserializer: D) -> Result<Volume, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let ml = if s.ends_with("ml") {
            s[..s.len() - 2].trim().parse().unwrap()
        } else if s.ends_with("L") {
            s[..s.len() - 1].trim().parse::<i32>().unwrap() * 1000
        } else {
            panic!("{}", s)
        };
        Ok(Volume { ml })
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, Copy)]
pub(crate) struct Weight {
    // s: String,
    pub g: i32,
}

impl<'de> serde::Deserialize<'de> for Weight {
    fn deserialize<D>(deserializer: D) -> Result<Weight, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let s = String::deserialize(deserializer)?;
        let g = if s.ends_with(" kg") {
            s[..s.len() - 3]
                .trim()
                .parse::<i32>()
                .map_err(|e| D::Error::custom(e.to_string()))?
                * 1000
        } else if s.ends_with(" mg") {
            // TODO: losing precision
            s[..s.len() - 2]
                .trim()
                .parse::<i32>()
                .map_err(|e| D::Error::custom(e.to_string()))?
                / 1000
        } else if s.ends_with(" g") {
            // TODO: losing precision
            s[..s.len() - 1]
                .trim()
                .parse::<i32>()
                .map_err(|e| D::Error::custom(e.to_string()))?
        } else {
            panic!("{}", s)
        };
        Ok(Weight { g })
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub(crate) struct PocketNormal {
    pub max_contains_volume: Volume,
    pub max_contains_weight: String,
    #[serde(default)] // drinking_hat
    pub moves: i32,
    #[serde(default)]
    pub flag_restriction: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub(crate) struct PocketAmmo {
    ammo_restriction: Map<String, i32>,
}

#[derive(Clone, Debug, serde::Deserialize, enum_as_inner::EnumAsInner)]
#[serde(untagged)]
pub(crate) enum PocketData {
    Normal(PocketNormal),
    Ammo(PocketAmmo),
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub(crate) struct Armor {
    pub covers: Vec<String>, // enum
    pub coverage: i32,
    pub encumbrance: i32,
    pub max_encumbrance: Option<i32>,
    pub warmth: i32,
    pub material_thickness: i32,
    pub environmental_protection: i32,
    #[serde(deserialize_with = "deserialize_vec_or_one")]
    pub material: Vec<String>,
    #[serde(skip_serializing)]
    pub pocket_data: Vec<PocketData>,
}

fn deserialize_vec_or_one<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Voo<U> {
        V(Vec<U>),
        O(U),
    }
    use serde::Deserialize;
    let voo = Voo::<T>::deserialize(deserializer)?;
    match voo {
        Voo::V(v) => Ok(v),
        Voo::O(o) => Ok(vec![o]),
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct Generic {}
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct Tool {}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
enum GenericItemEnum {
    #[serde(rename = "ARMOR")]
    Armor(Armor),
    #[serde(rename = "COMESTIBLE")]
    Comestible(Comestible),
    #[serde(rename = "GENERIC")]
    Generic(Generic),
    #[serde(rename = "TOOL")]
    Tool(Tool),
}
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct GenericItemHeader {
    #[serde(rename = "type")]
    pub typ: String, // enum
    pub id: String,
    pub name: Name,
    #[serde(default)]
    pub flags: Vec<String>,
    pub volume: Option<Volume>, // bone
    pub weight: Option<Weight>, // bone
    pub category: Option<String>,
    #[serde(default)]
    pub cutting: i32,
    #[serde(default)]
    pub bashing: i32,
}

#[derive(serde::Deserialize)]
struct GenericItemRaw {
    #[serde(flatten)]
    header: GenericItemHeader,
    #[serde(flatten)]
    inner: serde_json::Value,
}

impl From<GenericItemRaw> for CataItem {
    fn from(mut raw: GenericItemRaw) -> CataItem {
        raw.inner.as_object_mut().unwrap().insert(
            "type".to_string(),
            serde_json::to_value(&raw.header.typ).unwrap(),
        );
        // println!("{}", serde_json::to_string_pretty(&raw.inner).unwrap());
        let inner: GenericItemEnum = serde_json::from_value(raw.inner.clone())
            .unwrap_or_else(|e| panic!("{}: {:#?}", e, raw.inner));
        CataItem {
            header: raw.header,
            inner,
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(from = "GenericItemRaw")]
pub(crate) struct CataItem {
    pub header: GenericItemHeader,
    inner: GenericItemEnum,
}
macro_rules! cataitem_as {
    ($accessor_name:ident, $enum_variant:tt, $typ:ty) => {
        impl CataItem {
            pub(crate) fn $accessor_name(&self) -> &$typ {
                use GenericItemEnum::*;
                match &self.inner {
                    $enum_variant(inner) => inner,
                    z => panic!("Not a {} item - {:?}", stringify!($typ), z),
                }
            }
        }
    };
}
#[allow(unused_macros)]
macro_rules! cataitem_as_mut {
    ($accessor_name:ident, $enum_variant:tt, $typ:ty) => {
        impl CataItem {
            pub(crate) fn $accessor_name(&mut self) -> &mut $typ {
                use GenericItemEnum::*;
                match &mut self.inner {
                    $enum_variant(inner) => inner,
                    z => panic!("Not a {} item - {:?}", stringify!($typ), z),
                }
            }
        }
    };
}
cataitem_as!(as_armor, Armor, Armor);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ItemGroupItemType {
    Item,
    Group,
}

#[derive(Debug, Clone)]
pub(crate) struct ItemGroupItem {
    pub typ: ItemGroupItemType,
    pub id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct ItemGroup {
    pub id: String,
    pub subtype: Option<String>,
    #[serde(alias = "entries")] // not actually alias but whatever
    #[serde(deserialize_with = "deserialize_items_in_itemgroup")]
    #[serde(default)]
    pub items: Vec<(ItemGroupItem, i32)>,
}
fn deserialize_items_in_itemgroup<'de, D>(
    deserializer: D,
) -> Result<Vec<(ItemGroupItem, i32)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "kebab-case")]
    // #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct ItemVerbose {
        item: String,
        prob: Option<i32>,
        container_item: Option<String>,
        charges_min: Option<i32>,
        charges_max: Option<i32>,
        count_min: Option<i32>,
        count_max: Option<i32>,
    }

    #[derive(serde::Deserialize)]
    struct DistributionDummy {}
    #[derive(serde::Deserialize)]
    struct Scary {
        #[allow(dead_code)]
        distribution: Option<Vec<DistributionDummy>>,
        #[allow(dead_code)]
        collection: Option<Vec<DistributionDummy>>,
    }
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum ItemOrGroup {
        Item(String, Option<i32>),
        Group { group: String, prob: Option<i32> },
        ItemVerbose(ItemVerbose),
        Scary(Scary),
    }
    use serde::Deserialize;
    let raw: Vec<ItemOrGroup> = Vec::<ItemOrGroup>::deserialize(deserializer)?;
    let out = raw
        .into_iter()
        .filter_map(|iog| match iog {
            ItemOrGroup::Item(s, i)
            | ItemOrGroup::ItemVerbose(ItemVerbose {
                item: s, prob: i, ..
            }) => Some((
                ItemGroupItem {
                    typ: ItemGroupItemType::Item,
                    id: s,
                },
                i.unwrap_or(1000),
            )),
            ItemOrGroup::Group { group: s, prob: i } => Some((
                ItemGroupItem {
                    typ: ItemGroupItemType::Group,
                    id: s,
                },
                i.unwrap_or(1000),
            )),
            ItemOrGroup::Scary(_) => None,
        })
        .collect();
    Ok(out)
}
