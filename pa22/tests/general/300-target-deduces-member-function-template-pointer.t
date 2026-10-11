struct engine
{
  template<class T>
  void run(int, T&&);
};

using pointer = void (engine::*)(int, long&&);

void consume(pointer);

int main()
{
  consume(&engine::run);
}
