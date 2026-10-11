struct sigaction;

int f()
{
  return sizeof(struct sigaction);
}

struct sigaction {
  int ignored;
};
