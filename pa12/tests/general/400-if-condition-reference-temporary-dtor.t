int destroyed;

struct base {
  operator bool() const { return false; }
};

struct object : base {
  ~object() { ++destroyed; }
};

int main() {
  if(const base& value = object()) {}
  else if(destroyed) return 1;
  return destroyed != 1;
}
