struct params {
  int* value;
};

struct algorithm {
  int marker;
  algorithm(params);
};

int value;

int main()
{
  algorithm instance{{&value}};
  algorithm copied{{instance}};
}
