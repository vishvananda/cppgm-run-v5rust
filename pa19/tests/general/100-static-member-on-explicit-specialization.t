// HHC-140
template<class T>
struct char_traits {
  typedef T char_type;
  static int compare(const char_type*, const char_type*, int) noexcept;
};

template<>
struct char_traits<char16_t> {
  typedef char16_t char_type;
  static int compare(const char16_t*, const char16_t*, int) noexcept;
};

inline int char_traits<char16_t>::compare(const char_type*,
                                          const char_type*,
                                          int) noexcept {
  return 0;
}
