// Unevaluated scalar list casts visit their elements, even without list-node facts.
typedef int I;
int throwing();
int safe() noexcept;
int with_default(int = {throwing()}) noexcept;
int specification() noexcept(noexcept(int{throwing()}));
int main() {
    return noexcept(int{throwing()}) + noexcept(int{int{throwing()}})
        + noexcept(I{throwing()}) + noexcept(int{safe()})
        + noexcept(int{}) + noexcept(with_default()) + noexcept(specification());
}
