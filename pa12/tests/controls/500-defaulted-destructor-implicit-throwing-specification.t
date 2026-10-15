// expect exit 0
int destroyed;
struct Base {
  ~Base() { ++destroyed; }
};
struct Member {
  ~Member() noexcept(false) { destroyed += 10; }
};
struct Owner : Base {
  Member items[2];
  ~Owner() noexcept(false);
};
Owner::~Owner() = default;

int main()
{
  { Owner owner; }
  return destroyed == 21 ? 0 : 1;
}
