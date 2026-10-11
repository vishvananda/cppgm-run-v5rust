// VALIDATION: compile-pass
// N3485 focus: 11.4 [class.protected], 14.5.4 [temp.friend]

struct helper_collection
{
  int value;
};

template<class Archive>
struct interface_oarchive
{
  int get_helper(Archive * archive)
  {
    helper_collection & hc = archive->get_helper_collection();
    return hc.value;
  }
};

struct basic_oarchive
{
  helper_collection helper;

protected:
  helper_collection & get_helper_collection()
  {
    return helper;
  }
};

template<class Archive>
struct common_oarchive : public basic_oarchive
{
  friend class interface_oarchive<Archive>;
};

template<class Archive>
struct basic_text_oarchive : public common_oarchive<Archive>
{
  friend class interface_oarchive<Archive>;
};

template<class Archive>
struct text_oarchive_impl : public basic_text_oarchive<Archive>
{
  friend class interface_oarchive<Archive>;
};

struct text_oarchive : public text_oarchive_impl<text_oarchive>
{
};

int main()
{
  text_oarchive ar;
  ar.helper.value = 11;
  interface_oarchive<text_oarchive> iface;
  return iface.get_helper(&ar) == 11 ? 0 : 1;
}
