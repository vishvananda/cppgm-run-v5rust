template<class T>
void sink(T&&) {}

int main()
{
  sink([](int) {});
}
