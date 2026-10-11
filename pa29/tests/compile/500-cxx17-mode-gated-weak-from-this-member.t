// weak_from_this is not part of the C++11 library API. PA29 supports the later
// language mode because hosted STL headers expose mode-gated declarations.
template<class T>
struct enable_shared_from_this_like {
#if __cplusplus >= 201703L
  int weak_from_this() const { return 0; }
#endif
};

void use(const enable_shared_from_this_like<int> & value)
{
  value.weak_from_this();
}
