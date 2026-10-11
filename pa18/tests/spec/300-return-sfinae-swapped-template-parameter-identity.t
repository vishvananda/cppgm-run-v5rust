// N3485 focus: 14.5.6.1 [temp.over.link] return-type SFINAE patterns use
// template-parameter position, so these left/right overloads are distinct.

template<bool Condition, class T = void>
struct enable_if
{
};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

struct path
{
};

struct source
{
};

template<class T>
struct is_path
{
  static const bool value = false;
};

template<>
struct is_path<path>
{
  static const bool value = true;
};

template<class T>
struct is_source
{
  static const bool value = false;
};

template<>
struct is_source<source>
{
  static const bool value = true;
};

template<class Path, class Source>
typename enable_if<is_path<Path>::value && is_source<Source>::value, bool>::type
equal(Path const&, Source const&)
{
  return true;
}

template<class Source, class Path>
typename enable_if<is_path<Path>::value && is_source<Source>::value, bool>::type
equal(Source const&, Path const&)
{
  return true;
}

int main()
{
  path p;
  source s;
  return equal(p, s) && equal(s, p) ? 0 : 1;
}
