volatile int calls = 0;
int next() { ++calls; return calls; }

int main()
{
  int first[3] = {1, 2};
  int second[3] = {1, 2};
  int dynamic[3] = {4, next(), next()};
  volatile int observed[2] = {5, 6};
  first[0] = 9;
  return first != second && first[0] == 9 && second[0] == 1 &&
         second[1] == 2 && second[2] == 0 &&
         dynamic[0] == 4 && dynamic[1] == 1 && dynamic[2] == 2 &&
         calls == 2 && observed[0] == 5 && observed[1] == 6 ? 0 : 1;
}
