struct alignas(1) UnderAligned
{
  int value;
};

int main()
{
  UnderAligned object = {0};
  return object.value;
}
