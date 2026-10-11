// VALIDATION: compile-pass
template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> { typedef T type; };

template<class Allocator>
struct buffer {
  template<bool Mutable>
  struct range {
    range() = delete;
    range(const range&) = default;

    template<bool M = Mutable,
             class = typename enable_if<!M>::type>
    range(const range<true>&);
  };

  typedef range<true> mutable_range;
  typedef range<false> const_range;
};

typedef buffer<int> concrete_buffer;
typedef buffer<long> other_buffer;
static_assert(__is_convertible(concrete_buffer::mutable_range,
                               concrete_buffer::const_range), "");
static_assert(!__is_convertible(other_buffer::mutable_range,
                                concrete_buffer::const_range), "");

concrete_buffer::const_range convert(
    const concrete_buffer::mutable_range& source) {
  return source;
}
