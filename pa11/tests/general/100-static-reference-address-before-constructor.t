int values[3] = {3, 5, 7};
extern int *middle;
extern int &reference;
extern int &alias;

struct Observer
{
  bool valid;
  Observer() : valid(middle == &values[1] &&
                     &reference == &values[1] && &alias == &values[1]) {}
};

Observer observer;
int *middle = &values[1];
int &reference = values[1];
int &alias = reference;

int *current = &values[0];
int *advance() { current = &values[2]; return current; }
int *advanced = advance();
int *copied = current;
int &through_pointer = *current;

int main()
{
  alias = 11;
  return observer.valid && values[1] == 11 &&
         advanced == &values[2] && copied == advanced &&
         &through_pointer == advanced ? 0 : 1;
}
