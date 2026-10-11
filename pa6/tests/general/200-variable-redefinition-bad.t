// N3485 3.2p1: int x = 1; int x; defines x twice; must be rejected
int x = 1;
int x;
int main() { return x; }
