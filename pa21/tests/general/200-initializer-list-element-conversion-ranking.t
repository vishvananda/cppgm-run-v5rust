namespace std {
template<class T>
class initializer_list {
  const T * first;
  unsigned long count;

  initializer_list(const T * begin, unsigned long size) :
    first(begin), count(size)
  {}

public:
  initializer_list() : first(0), count(0) {}
};
}

struct IntResult {};
struct UnsignedResult {};

struct Box {
  IntResult operator=(std::initializer_list<int>);
  UnsignedResult operator=(std::initializer_list<unsigned int>);
};

template<typename T, typename U>
struct IsSame {
  static const bool value = false;
};

template<typename T>
struct IsSame<T, T> {
  static const bool value = true;
};

Box& get_box();

static_assert(IsSame<decltype(get_box() = {1}), IntResult>::value,
              "exact initializer-list element conversion should win");
static_assert(IsSame<decltype(get_box() = {1u}), UnsignedResult>::value,
              "exact unsigned initializer-list element conversion should win");
