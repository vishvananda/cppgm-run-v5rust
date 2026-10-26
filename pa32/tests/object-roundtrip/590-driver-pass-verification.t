template<class T> T step(T value) { return value + 1; }
struct Source { virtual long value() const = 0; };
long sum(Source* source, long count) {
    long result = 0;
    try {
        for (long i = 0; i < count; ++i) result += step(source->value());
    } catch (int code) { result += code; }
    return result;
}
