typedef decltype(sizeof(0)) size_t;
typedef long max_align_t;

namespace std {
  inline namespace __1 {
    template<class It>
    struct __wrap_iter {};

    template<class T1, class T2>
    inline bool operator<(const __wrap_iter<T1>&, const __wrap_iter<T2>&) { return true; }

    template<class T1, class T2>
    inline bool operator!=(const __wrap_iter<T1>&, const __wrap_iter<T2>&) { return true; }

    template<class T1>
    inline bool operator>(const __wrap_iter<T1>&, const __wrap_iter<T1>&) { return false; }

    using ::max_align_t __attribute__((__using_if_exists__));

    namespace __math {
      template<class = int>
      int fpclassify(double);
    }

    using std::__math::fpclassify;
  }
}

template<class C>
struct X {
  C& c;

  [[gnu::always_inline]] X(C& in) : c(in) {}
};

template<class T>
struct Y {
  using type [[gnu::nodebug]] = __remove_all_extents(T);
};

template<class T>
using YPtr [[gnu::nodebug]] = __add_pointer(T);

enum class E { V };

template<class T>
struct Z {
  typedef __underlying_type(T) type;
};

template<class _Tp>
inline __attribute__((__visibility__("hidden"))) __attribute__((__always_inline__))
_Tp* __libcpp_allocate(unsigned long __n, [[__maybe_unused__]] size_t __align = alignof(_Tp)) {
  (void)__align;
  return (_Tp*)0;
}

int main() {
  return 0;
}
