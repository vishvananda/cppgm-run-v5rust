// expect reject
struct Member {
private:
  ~Member() {}
};
struct Owner {
  Member items[2];
  ~Owner();
};
Owner::~Owner() = default;

int main() { return 0; }
