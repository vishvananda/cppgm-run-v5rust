struct Arg {};

struct Target {
  template <class T>
  Target(T&&) {
    static_assert(sizeof(T) == 0,
                  "constructor body should stay uninstantiated for worse conversion candidates");
  }
};

char accept(Target);
long accept(Arg);

static_assert(sizeof(accept(*(Arg*)0)) == sizeof(long), "");
