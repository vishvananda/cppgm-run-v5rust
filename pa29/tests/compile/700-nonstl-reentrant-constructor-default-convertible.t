template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

struct source {};

struct header {
  header() {}
  header(header const&) {}

  template<class Arg,
           class = typename enable_if<!__is_convertible(Arg, header)>::type>
  explicit header(Arg&&) {}
};

static_assert(!__is_convertible(source, header), "");
static_assert(__is_constructible(header, source), "");

int main() {
  header original;
  header copy(original);
  source input;
  header direct(input);
  (void)copy;
  (void)direct;
}
