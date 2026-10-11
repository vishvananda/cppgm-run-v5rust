#if __has_builtin(__reference_constructs_from_temporary)
template<class T, class U>
inline const bool reference_temporary_v = __reference_constructs_from_temporary(T, U);
#else
template<class T, class U>
inline const bool reference_temporary_v = __reference_binds_to_temporary(T, U);
#endif

struct S {};

template<class H>
struct leaf {
  template<class T>
  explicit leaf(T&& value) {
    static_assert(!reference_temporary_v<H, T&&>);
    (void)value;
  }
};

int main()
{
  S s;
  leaf<S&&> value(static_cast<S&&>(s));
  (void)value;
  return 0;
}
