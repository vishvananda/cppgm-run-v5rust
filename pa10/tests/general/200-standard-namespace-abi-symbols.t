namespace std {
  int value = 3;
  struct Tag { int value; };
  const Tag object = {4};
  int read(const Tag& source) { return source.value; }
  static int internal() { return value; }
}
namespace Other { namespace std { int read() { return 5; } } }
int main() { return std::read(std::object) + std::internal() + Other::std::read(); }
