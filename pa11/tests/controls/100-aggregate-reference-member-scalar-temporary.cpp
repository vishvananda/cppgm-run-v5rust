// N3485 8.5.1p2, 8.5.3p5, 12.2p5: an aggregate's const int& member initialized
// from a scalar prvalue binds a temporary that lives as long as the reference;
// expect exit 0
struct R { const int& r; int k; };
int in = 41;
R gr = { in + 1, 7 };
int main() { R lr = { in + 2, 7 }; return (gr.r == 42 && lr.r == 43) ? 0 : 1; }
