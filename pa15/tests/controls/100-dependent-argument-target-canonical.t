// expect exit 0
// Convert a known later target while preserving the unrelated dependent tag.
template<class Tag, class T, T Value> struct argument {
  static const int value = Value;
};
template<class Tag> struct holder {
  typedef argument<Tag, bool, 1> first;
  typedef argument<Tag, bool, true> second;
};
template<class T> struct identity { static const int value = 0; };
template<> struct identity<argument<int, bool, true>> {
  static const int value = 1;
};
int main() {
  return holder<int>::first::value != 1 ||
         identity<holder<int>::second>::value != 1;
}
