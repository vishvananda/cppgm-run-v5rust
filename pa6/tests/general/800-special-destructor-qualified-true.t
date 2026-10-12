// Frozen types-mode special-member declaration responsibility regression.
struct C{~C()noexcept;};C::~C(){}
