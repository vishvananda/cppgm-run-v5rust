int copies = 0;

struct Counted {
  int value;

  Counted(int v) : value(v) {}
  Counted(const Counted &other) : value(other.value) { ++copies; }
};

struct Holder {
  Counted value;
};

const Holder a = {Counted(3)};
const Holder b = {Counted(4)};

const Holder table[] = {a, b};

int main()
{
  return table[0].value.value == 3 && table[1].value.value == 4 && copies == 2 ? 0 : 1;
}
