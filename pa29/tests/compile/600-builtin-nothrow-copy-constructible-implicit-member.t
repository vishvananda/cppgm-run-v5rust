struct maybe_throw_default {
  maybe_throw_default() {}
};

struct holder {
  maybe_throw_default value;
};

struct holder_with_initializer {
  maybe_throw_default value = maybe_throw_default();
};

static_assert(!__is_nothrow_constructible(holder),
              "default construction still observes the member constructor");
static_assert(__is_nothrow_constructible(holder, const holder&),
              "implicit copy construction uses member copy construction");
static_assert(!__is_nothrow_constructible(holder_with_initializer),
              "default construction still observes default member initializers");
static_assert(__is_nothrow_constructible(holder_with_initializer,
                                         const holder_with_initializer&),
              "implicit copy construction ignores default member initializers");

int main() {
  return 0;
}
