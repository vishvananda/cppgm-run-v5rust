int alive;
int destroyed;

struct Payload {
    long long first;
    long long second;
    Payload(long long x, long long y) : first(x), second(y) { ++alive; }
    ~Payload() { --alive; ++destroyed; }
    long long sum() const { return first + second; }
};

int main() {
    {
        const Payload& bound = Payload(10, 20);
        long long sentinel = 91;
        if (bound.sum() != 30 || sentinel != 91 || alive != 1 || destroyed != 0)
            return 1;
        {
            Payload&& second = Payload{31, 42};
            if (second.sum() != 73 || bound.sum() != 30 || alive != 2)
                return 2;
        }
        if (alive != 1 || destroyed != 1 || bound.sum() != 30) return 3;
    }
    return alive != 0 || destroyed != 2;
}
