# host O0 simple inline constructor symbol is retained
# split
struct HostSimpleCtor {
  int value;
  HostSimpleCtor() : value(3) {}
};

int main()
{
  HostSimpleCtor value;
  return value.value == 3 ? 0 : 1;
}
