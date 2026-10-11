// VALIDATION: compile-pass
// N3485 focus: 14.2 [temp.names], 14.8.1 [temp.arg.explicit]

template<class T>
int choose(T)
{
  return 1;
}

int choose(int)
{
  return 2;
}

int main()
{
  return choose<int>(0) == 1 ? 0 : 1;
}
