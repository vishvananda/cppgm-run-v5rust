// N3485 focus: 13.3.3.2 [over.ics.rank] ellipsis conversion ranking
void f(...);
int f(int*);

int x = f(0);
int main() { return x; }
