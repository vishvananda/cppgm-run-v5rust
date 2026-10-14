// The caller owns a by-address class parameter: it is still alive after the
// callee returns and is destroyed at the end of the caller's full-expression,
// in reverse order of construction with the other temporaries there, also in
// a namespace-scope dynamic initializer (Itanium C++ ABI 3.1.2.3; N3485
// 12.2p3).
int events[16];
int event_count = 0;

void note(int event) {
  if (event_count < 16) events[event_count++] = event;
}

struct Value {
  int id;
  Value(int v) : id(v) { note(v); }
  Value(const Value& other) : id(other.id + 10) { note(id); }
  ~Value() { note(-id); }
};

struct Guard {
  int id;
  Guard(int v) : id(v) { note(v); }
  ~Guard() { note(-id); }
};

bool take(Value value) {
  note(100 + value.id);
  return true;
}

bool seen(int v) {
  note(200 + v);
  return true;
}

bool global_taken = take(Value(40));

int main() {
  {
    Value source(1);
    bool taken = (Guard(2), take(source)) && seen(3);
    note(taken ? 300 : 301);
  }
  static const int expected[] = {
    40, 140, -40, 1, 2, 11, 111, 203, -11, -2, 300, -1
  };
  if (!global_taken || event_count != 12) return 100 + event_count;
  for (int i = 0; i < 12; ++i)
    if (events[i] != expected[i]) return 1 + i;
  return 0;
}
