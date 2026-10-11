struct Holder {
  volatile int values[2];
  volatile unsigned field : 3;
  unsigned adjacent : 3;
  Holder() : values{5, 6}, field(3), adjacent(4) {}
};
struct Aggregate {
  volatile unsigned field : 3;
  unsigned adjacent : 3;
};
int main() {
  Holder holder;
  Aggregate aggregate = {2, 5};
  ++aggregate.field;
  return holder.values[0] != 5 || holder.values[1] != 6 ||
    holder.field != 3 || holder.adjacent != 4 ||
    aggregate.field != 3 || aggregate.adjacent != 5;
}
