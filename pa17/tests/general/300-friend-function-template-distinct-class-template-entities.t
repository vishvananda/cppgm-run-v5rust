// Friend function-template overloads with identically spelled class-template
// arguments from different namespaces are distinct declarations.
namespace foreign {
template<class T> struct view {};
}

namespace home {
template<class T> struct view {};

struct stream {
  template<class U>
  friend int put(stream&, foreign::view<U> const&) { return 1; }

  template<class U>
  friend int put(stream&, view<U> const&) { return 2; }
};
}

int main()
{
  home::stream out;
  home::view<int> value;
  return put(out, value) == 2 ? 0 : 1;
}
