struct Payload {
  int value;
  constexpr Payload(int v) : value(v) {}
};

struct Adapted : Payload {
  constexpr Adapted(Payload p) : Payload(p) {}
};

constexpr Payload payload(7);
constexpr Adapted adapted(payload);

static_assert(adapted.value == 7, "");

int main() {
  return 0;
}
