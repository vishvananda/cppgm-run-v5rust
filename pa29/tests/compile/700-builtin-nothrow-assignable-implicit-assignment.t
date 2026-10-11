template <bool, class T = void>
struct enable_if {
};

template <class T>
struct enable_if<true, T> {
  typedef T type;
};

template <class T>
struct is_enum {
  static const bool value = false;
};

struct category;

struct error_code_like {
  struct data {
    int val;
    const category* cat;
  };

  union {
    data d1;
    unsigned char d2[16];
  };

  unsigned long flags;

  error_code_like() noexcept
      : d1(), flags(0) {
  }

  error_code_like(const error_code_like&, const void*) noexcept
      : d1(), flags(0) {
  }

  template <class Enum>
  typename enable_if<is_enum<Enum>::value, error_code_like>::type&
  operator=(Enum) noexcept;
};

static_assert(__is_assignable(error_code_like&, error_code_like&&),
              "implicit assignment is viable");
static_assert(__is_nothrow_assignable(error_code_like&, error_code_like&&),
              "implicit assignment is nothrow when subobject assignment is nothrow");

int main() {
  return 0;
}
