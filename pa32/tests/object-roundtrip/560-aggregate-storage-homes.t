struct Pair {
  long first;
  long second;
};

struct Odd {
  char bytes[9];
};

struct Large {
  long words[4];
};

extern Pair produce_pair();
extern Odd produce_odd();
extern void observe(const void *);

__attribute__((noinline)) long parameter_home(Pair value) {
  observe(&value);
  return value.second;
}

__attribute__((noinline)) long stack_parameter_home(Large value) {
  observe(&value);
  return value.words[3];
}

__attribute__((noinline)) long result_destination() {
  Pair value = produce_pair();
  observe(&value);
  return value.second;
}

__attribute__((noinline)) char padded_result_keeps_home() {
  Odd value = produce_odd();
  observe(&value);
  return value.bytes[8];
}
