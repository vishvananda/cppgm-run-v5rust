// expect exit 0
// Invisible namespace function declarations do not hide an existing class name.
namespace model {
struct inspect {};
struct Box {
  int value;
  friend int inspect(Box & b) { return b.value; }
};
int size() {
  inspect object;
  return sizeof(object);
}
}
int main() { return model::size() == 1 ? 0 : 1; }
