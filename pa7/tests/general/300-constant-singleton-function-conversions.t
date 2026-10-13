int function(int);
void consume(bool);
int choose(bool);
int choose(int (*)(int));
namespace associated
{
  struct type;
  void argument(type*);
  void dispatch(void (*)(type*));
}
constexpr bool value = bool(function);
int result()
{
  consume(function);
  dispatch(associated::argument);
  return value + choose(function);
}
