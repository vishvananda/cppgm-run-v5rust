// A static list's backing storage and its wrapper retain distinct identities.
// expect exit 0
namespace std { template<class T> class initializer_list; }
std::initializer_list<int> global = {3, 4};
std::initializer_list<int> empty = {};

int total(const std::initializer_list<int>& values) {
  int result = 0;
  for (auto item : values) result += item;
  return result;
}
int run() {
  static std::initializer_list<int> values = {5, 6};
  static std::initializer_list<int> none = {};
  const std::initializer_list<int>& local = {7, 8};
  return total(values) + total(none) + total(local);
}
int main() {
  return total(global) == 7 && total(empty) == 0 && run() == 26
      && run() == 26 ? 0 : 1;
}
