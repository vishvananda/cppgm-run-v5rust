// lowir.md storage=readonly: a const global states storage=readonly only when
// it is statically initialized and not thread-local. A dynamically initialized
// const scalar or array is set by the program's initialization function, and
// a thread-local one lives in thread storage; neither states storage=readonly,
// while a statically initialized const global beside them does.
int seed();
const int scalar = 3;
const int dynamic_scalar = seed();
const int dynamic_table[2] = {seed(), 2};
thread_local const int thread_scalar = 11;

int seed() { return 1; }

int main()
{
  int sum = scalar + dynamic_scalar + dynamic_table[0] + dynamic_table[1] + thread_scalar;
  return sum == 3 + 1 + 1 + 2 + 11 ? 0 : 1;
}
