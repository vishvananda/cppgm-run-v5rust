int choose(int);
long choose(long);
int (*pointer)(int);
int value;
int direct[sizeof(choose(1))];
int indirect[sizeof(pointer(1))];
int dereferenced[sizeof((*pointer)(1))];
int increment[sizeof(++value)];
int assignment[sizeof(value = 2)];
void local()
{
  int array[sizeof(choose(1))];
}
