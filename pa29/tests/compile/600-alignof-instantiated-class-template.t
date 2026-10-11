template <class T>
struct box {
  T value;
};

int main()
{
  return alignof(box<int>) > 0 ? 0 : 1;
}
