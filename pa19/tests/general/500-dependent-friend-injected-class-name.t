// PA17 focus: collecting an out-of-class member definition for a dependent
// class-template instantiation must not canonicalize friend class declarations
// that use the current class injected-name.

namespace mini {

typedef unsigned long size_t;

template<class T>
struct hash {
  typedef T argument_type;
};

template<class T, class D>
struct unique_ptr {
  typedef T *pointer;
};

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool... T>
struct all {
  static const bool value = true;
};

template<class Key, class Hash = hash<Key> >
struct has_enabled_hash {
  static const bool value = true;
};

template<class Type, class>
using enable_hash_helper_imp = Type;

template<class Type, class... Keys>
using enable_hash_helper =
    enable_hash_helper_imp<
        Type,
        typename enable_if<all<has_enabled_hash<Keys>::value...>::value>::type>;

template<class T, class D>
struct hash<enable_hash_helper<unique_ptr<T, D>, typename unique_ptr<T, D>::pointer> >;

template<size_t _Size>
class bitset;

template<size_t _Size>
struct hash<bitset<_Size> >;

template<size_t _Size>
class bitset {
public:
  bitset &operator&=(const bitset &rhs);

private:
  friend struct hash<bitset>;
};

template<size_t _Size>
inline bitset<_Size> &bitset<_Size>::operator&=(const bitset &rhs)
{
  (void)rhs;
  return *this;
}

template<size_t _Size>
struct hash<bitset<_Size> > {
  typedef bitset<_Size> argument_type;
};

}

int main()
{
  return 0;
}
