struct C {
  typedef int duration;
};

int clock();

void f()
{
  typedef C clock;
  clock::duration d;
}
