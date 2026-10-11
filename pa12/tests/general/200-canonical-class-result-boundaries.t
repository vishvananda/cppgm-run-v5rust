int destroyed;

struct Small {
    int value;
    Small(int x) : value(x) {}
};
typedef Small Alias;
Alias make_small(int x) { return Small(x); }

struct Empty {
    Empty() {}
    ~Empty() { ++destroyed; }
};
Empty make_empty() { return Empty(); }

struct Sixteen {
    long long first;
    long long second;
    Sixteen(long long x) : first(x), second(42) {}
    ~Sixteen() { ++destroyed; }
};
Sixteen make_sixteen(long long x) { return Sixteen(x); }

int main() {
    Small (*small)(int) = make_small;
    if (small(7).value != 7 || make_small(9).value != 9) return 1;
    Empty (*empty)() = make_empty;
    empty();
    if (destroyed != 1) return 2;
    make_empty();
    if (destroyed != 2) return 3;
    {
        Sixteen (*sixteen)(long long) = make_sixteen;
        Sixteen value = sixteen(31);
        if (value.first != 31 || value.second != 42) return 4;
    }
    return destroyed != 3;
}
