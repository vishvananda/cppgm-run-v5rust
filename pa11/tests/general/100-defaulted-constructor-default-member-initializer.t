struct holder
{
  int value = 7;
  holder() = default;
};

int main()
{
  holder h;
  return h.value == 7 ? 0 : 1;
}
