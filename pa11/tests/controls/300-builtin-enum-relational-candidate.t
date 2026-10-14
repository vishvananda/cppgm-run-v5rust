// N3485 13.3.1.2p3, 13.6p15: built-in operator<(E,E) is a candidate and beats
// user operator<(E,int) (promotion) for E < E; expect exit 0
enum E { A = 5, B = 2 };
bool operator<(E, int) { return true; }
int main() { E x = A, y = B; return (x < y) ? 1 : 0; }
