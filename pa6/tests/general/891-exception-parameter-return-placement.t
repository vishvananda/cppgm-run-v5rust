// N3485 15.4p2: a specified function may be a parameter or return type.
void accept(void (*callback)() noexcept);
void (*factory())() noexcept;
