struct codecvt {
};

struct string {
  int value;
  string() : value(7) {}
};

struct path {
  typedef codecvt codecvt_type;

  template<class String>
  String string(codecvt_type const& cvt) const;

  ::string const& string(codecvt_type const& cvt) const;
  ::string storage;
};

inline ::string const& path::string(codecvt_type const& cvt) const
{
  return storage;
}

template<>
inline ::string path::string<::string>(codecvt_type const& cvt) const
{
  return string(cvt);
}

int main()
{
  codecvt cvt;
  path p;
  return p.string<::string>(cvt).value == 7 ? 0 : 1;
}
