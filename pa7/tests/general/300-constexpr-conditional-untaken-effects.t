int read();
int value;
int* pointer;
constexpr int call = true ? 1 : read();
constexpr int assignment = false ? (value = 9) : 2;
constexpr int increment = true ? 3 : ++value;
constexpr int indirection = true ? 4 : *pointer;
constexpr bool conversion = true ? true : bool(pointer);
constexpr unsigned long common = true ? 5 : 9UL;
int result()
{
  return call + assignment + increment + indirection + conversion + common +
         __builtin_constant_p(true ? 6 : read());
}
