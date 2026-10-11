namespace std {
template<class T> T&& declval();
template<class T> T&& forward(T&);
template<class T> T&& forward(T&&);
}

template<class A, class B, class C>
struct Key {};

template<class _KeyT, class _WithKey, class _WithoutKey, class... _Args>
auto __try_key_extraction(_WithKey __with_key, _WithoutKey __without_key, _Args&&... __args)
  -> decltype(std::declval<_WithoutKey>()(std::declval<_Args>()...))
{
  return __without_key(std::forward<_Args>(__args)...);
}

template<class _Tp>
struct Table {
  using key_type = _Tp;

  template<class... _Args>
  int emplace(_Args&&... __args)
  {
    return __try_key_extraction<key_type>(
        [this](const key_type& __key, _Args&&... __args2) { return 1; },
        [](_Args&&... __args2) { return 0; },
        std::forward<_Args>(__args)...);
  }
};

int main()
{
  Table<Key<int, char, long> > t;
  return t.emplace("x");
}
