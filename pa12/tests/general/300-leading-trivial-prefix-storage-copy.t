union Payload {
  long long bits;
  int code;
};

struct Tail {
  int value;
  Tail() : value(0) {}
  Tail(const Tail& o) : value(o.value) {}
  Tail(Tail&& o) : value(o.value) {}
  Tail& operator=(const Tail& o) { value = o.value; return *this; }
  Tail& operator=(Tail&& o) { value = o.value; return *this; }
  ~Tail() {}
};

struct Box {
  int tag;
  Payload payload;
  Tail tail;
};

int main() {
  Box a;
  Box b = a;
  b = a;
  Box c = static_cast<Box&&>(a);
  c = static_cast<Box&&>(b);
  return 0;
}
