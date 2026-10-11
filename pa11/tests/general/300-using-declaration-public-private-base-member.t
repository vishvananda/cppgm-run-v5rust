struct layout {
  int *begin() { return 0; }
  const int *begin() const { return 0; }
};

struct buffer : private layout {
 public:
  using layout::begin;
};

struct owner {
  int *call_begin() {
    return storage.begin();
  }

  buffer storage;
};

int main() {
  owner value;
  return value.call_begin() == 0 ? 0 : 1;
}
