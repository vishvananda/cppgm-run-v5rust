// expect reject
// Explicitness is checked before deletedness after ranking this best candidate.
struct Item {
    explicit Item(int) = delete;
    Item(double);
};
int main() { Item item = {1}; return 0; }
