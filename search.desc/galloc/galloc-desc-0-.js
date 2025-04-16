searchState.loadedDescShard("galloc", 0, "A pointer type that uniquely owns a heap allocation of …\nThe associated error which can be returned from parsing.\nParse a value from a string\nThe resulting type after obtaining ownership.\nA UTF-8–encoded, growable string.\nA generalization of <code>Clone</code> to borrowed data.\nA trait for converting a value to a <code>String</code>.\nA contiguous growable array type, written as <code>Vec&lt;T&gt;</code>, short …\nA double-ended queue implemented with a growable ring …\nReturns a reference to the underlying allocator.\nReturns a reference to the underlying allocator.\nReturns a reference to the underlying allocator.\nMoves all the elements of <code>other</code> into <code>self</code>, leaving <code>other</code> …\nMoves all the elements of <code>other</code> into <code>self</code>, leaving <code>other</code> …\nReturns a byte slice of this <code>String</code>’s contents.\nReturns a raw mutable pointer to the <code>Box</code>’s contents.\nReturns a raw mutable pointer to the vector’s buffer, or …\nExtracts a mutable slice of the entire vector.\nReturns a pair of slices which contain, in order, the …\nConverts a <code>String</code> into a mutable string slice.\nReturns a mutable reference to the contents of this <code>String</code>.\nReturns a <code>NonNull</code> pointer to the vector’s buffer, or a …\nReturns a raw pointer to the <code>Box</code>’s contents.\nReturns a raw pointer to the vector’s buffer, or a …\nExtracts a slice containing the entire vector.\nReturns a pair of slices which contain, in order, the …\nExtracts a string slice containing the entire <code>String</code>.\nConverts to <code>Box&lt;[T], A&gt;</code>.\nConverts to <code>Box&lt;T, A&gt;</code>.\nProvides a reference to the back element, or <code>None</code> if the …\nProvides a mutable reference to the back element, or <code>None</code> …\nBinary searches this <code>VecDeque</code> for a given element. If the …\nBinary searches this <code>VecDeque</code> with a comparator function.\nBinary searches this <code>VecDeque</code> with a key extraction …\nReturns the number of elements the deque can hold without …\nReturns the total number of elements the vector can hold …\nReturns this <code>String</code>’s capacity, in bytes.\nClears the deque, removing all values.\nClears the vector, removing all values.\nTruncates this <code>String</code>, removing all contents.\nReturns a new box with a <code>clone()</code> of this box’s contents.\nCopies <code>source</code>’s contents into <code>self</code> without creating a …\nCopies <code>source</code>’s contents into <code>self</code> without creating a …\nOverwrites the contents of <code>self</code> with a clone of the …\nOverwrites the contents of <code>self</code> with a clone of the …\nClones the contents of <code>source</code> into <code>self</code>.\nUses borrowed data to replace owned data, usually by …\nReturns <code>true</code> if the deque contains an element equal to the …\nRemoves consecutive repeated elements in the vector …\nRemoves all but the first of consecutive elements in the …\nRemoves all but the first of consecutive elements in the …\nCreates a <code>Box&lt;T&gt;</code>, with the <code>Default</code> value for T.\nCreates an empty deque.\nCreates an empty <code>Vec&lt;T&gt;</code>.\nCreates an empty <code>String</code>.\nAttempts to downcast the box to a concrete type.\nAttempts to downcast the box to a concrete type.\nAttempts to downcast the box to a concrete type.\nDowncasts the box to a concrete type.\nDowncasts the box to a concrete type.\nDowncasts the box to a concrete type.\nRemoves the specified range from the deque in bulk, …\nRemoves the specified range from the vector in bulk, …\nRemoves the specified range from the string in bulk, …\nClones and appends all elements in a slice to the <code>Vec</code>.\nGiven a range <code>src</code>, clones a slice of elements in that …\nCopies elements from <code>src</code> range to the end of the string.\nCreates an iterator which uses a closure to determine if …\nReturns the contents of the “front” slice as returned …\nCreates a <code>String</code> using interpolation of runtime …\nConverts a <code>String</code> into a box of dyn <code>Error</code> + <code>Send</code> + <code>Sync</code>.\nConverts a <code>Cow</code> into a box of dyn <code>Error</code>.\nCopies the string into a newly allocated Box&lt;OsStr&gt;.\nCopies the string into a newly allocated Box&lt;OsStr&gt;.\nReturns the argument unchanged.\nConverts a vector into a boxed slice.\nCreates a boxed <code>Path</code> from a clone-on-write pointer.\nCreates a boxed <code>Path</code> from a reference.\nCreates a boxed <code>Path</code> from a reference.\nConverts an <code>OsString</code> into a Box&lt;OsStr&gt; without copying or …\nConverts a <code>T</code> into a <code>Box&lt;T&gt;</code>\nConverts a <code>&amp;[T]</code> into a <code>Box&lt;[T]&gt;</code>\nConverts a <code>&amp;mut [T]</code> into a <code>Box&lt;[T]&gt;</code>\nConverts a <code>Cow&lt;&#39;_, [T]&gt;</code> into a <code>Box&lt;[T]&gt;</code>\nConverts a <code>&amp;str</code> into a <code>Box&lt;str&gt;</code>\nConverts a <code>&amp;mut str</code> into a <code>Box&lt;str&gt;</code>\nConverts a <code>Cow&lt;&#39;_, str&gt;</code> into a <code>Box&lt;str&gt;</code>\nConverts a <code>Box&lt;str&gt;</code> into a <code>Box&lt;[u8]&gt;</code>\nConverts a <code>[T; N]</code> into a <code>Box&lt;[T]&gt;</code>\nConverts a type of <code>Error</code> into a box of dyn <code>Error</code>.\nConverts a type of <code>Error</code> + <code>Send</code> + <code>Sync</code> into a box of dyn …\nConverts a <code>PathBuf</code> into a Box&lt;Path&gt;.\nConverts the given <code>String</code> to a boxed <code>str</code> slice that is …\nConverts a <code>String</code> into a box of dyn <code>Error</code>.\nConverts a <code>str</code> into a box of dyn <code>Error</code> + <code>Send</code> + <code>Sync</code>.\nConverts a <code>str</code> into a box of dyn <code>Error</code>.\nConverts a <code>Cow</code> into a box of dyn <code>Error</code> + <code>Send</code> + <code>Sync</code>.\nConverts a <code>CString</code> into a Box&lt;CStr&gt; without copying or …\nConverts a <code>Cow&lt;&#39;a, CStr&gt;</code> into a <code>Box&lt;CStr&gt;</code>, by copying the …\nConverts a <code>&amp;mut CStr</code> into a <code>Box&lt;CStr&gt;</code>, by copying the …\nConverts a <code>&amp;CStr</code> into a <code>Box&lt;CStr&gt;</code>, by copying the contents …\nConverts a <code>Cow&lt;&#39;a, OsStr&gt;</code> into a Box&lt;OsStr&gt;, by copying …\nTurn a <code>Vec&lt;T&gt;</code> into a <code>VecDeque&lt;T&gt;</code>.\nReturns the argument unchanged.\nConverts a <code>[T; N]</code> into a <code>VecDeque&lt;T&gt;</code>.\nConverts a <code>BinaryHeap&lt;T&gt;</code> into a <code>Vec&lt;T&gt;</code>.\nTurn a <code>VecDeque&lt;T&gt;</code> into a <code>Vec&lt;T&gt;</code>.\nConverts a boxed slice into a vector by transferring …\nConverts a clone-on-write slice into a vector.\nConverts a <code>CString</code> into a Vec&lt;u8&gt;.\nReturns the argument unchanged.\nAllocates a <code>Vec&lt;T&gt;</code> and moves <code>s</code>’s items into it.\nAllocates a <code>Vec&lt;T&gt;</code> and fills it by cloning <code>s</code>’s items.\nAllocates a <code>Vec&lt;T&gt;</code> and fills it by cloning <code>s</code>’s items.\nAllocates a <code>Vec&lt;T&gt;</code> and fills it by cloning <code>s</code>’s items.\nAllocates a <code>Vec&lt;T&gt;</code> and fills it by cloning <code>s</code>’s items.\nAllocates a <code>Vec&lt;u8&gt;</code> and fills it with a UTF-8 string.\nConverts the given <code>String</code> to a vector <code>Vec</code> that holds …\nAllocates an owned <code>String</code> from a single character.\nConverts the given boxed <code>str</code> slice to a <code>String</code>. It is …\nReturns the argument unchanged.\nConverts a clone-on-write string to an owned instance of …\nConverts a <code>&amp;String</code> into a <code>String</code>.\nConverts a <code>&amp;str</code> into a <code>String</code>.\nConverts a <code>&amp;mut str</code> into a <code>String</code>.\nConstructs a box from a <code>NonNull</code> pointer.\nConstructs a box from a <code>NonNull</code> pointer in the given …\nCreates a <code>Vec&lt;T&gt;</code> directly from a <code>NonNull</code> pointer, a …\nCreates a <code>Vec&lt;T, A&gt;</code> directly from a <code>NonNull</code> pointer, a …\nConstructs a box from a raw pointer.\nConstructs a box from a raw pointer in the given allocator.\nCreates a <code>Vec&lt;T&gt;</code> directly from a pointer, a length, and a …\nCreates a new <code>String</code> from a pointer, a length and a …\nCreates a <code>Vec&lt;T, A&gt;</code> directly from a pointer, a length, a …\nParses a string <code>s</code> to return a value of this type.\nDecode a UTF-16–encoded vector <code>v</code> into a <code>String</code>, …\nDecode a UTF-16–encoded slice <code>v</code> into a <code>String</code>, replacing …\nDecode a UTF-16BE–encoded vector <code>v</code> into a <code>String</code>, …\nDecode a UTF-16BE–encoded slice <code>v</code> into a <code>String</code>, …\nDecode a UTF-16LE–encoded vector <code>v</code> into a <code>String</code>, …\nDecode a UTF-16LE–encoded slice <code>v</code> into a <code>String</code>, …\nConverts a vector of bytes to a <code>String</code>.\nConverts a slice of bytes to a string, including invalid …\nConverts a <code>Vec&lt;u8&gt;</code> to a <code>String</code>, substituting invalid UTF-8 …\nConverts a vector of bytes to a <code>String</code> without checking …\nProvides a reference to the front element, or <code>None</code> if the …\nProvides a mutable reference to the front element, or <code>None</code> …\nProvides a reference to the element at the given index.\nProvides a mutable reference to the element at the given …\nInserts an element at <code>index</code> within the deque, shifting all …\nInserts an element at position <code>index</code> within the vector, …\nInserts a character into this <code>String</code> at a byte position.\nInserts a string slice into this <code>String</code> at a byte position.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nConverts the boxed slice into a boxed array.\nConverts a <code>Box&lt;T&gt;</code> into a <code>Box&lt;[T]&gt;</code>\nConverts the vector into <code>Box&lt;[T]&gt;</code>.\nConverts this <code>String</code> into a Box&lt;str&gt;.\nConverts a <code>String</code> into a byte vector.\nConverts a <code>String</code> into an iterator over the <code>char</code>s of the …\nTakes a <code>Vec&lt;[T; N]&gt;</code> and flattens it into a <code>Vec&lt;T&gt;</code>.\nConsumes the <code>Box</code>, returning the wrapped value.\nConsumes the deque into a front-to-back iterator yielding …\nCreates a consuming iterator, that is, one that moves each …\nConsumes the <code>Box</code>, returning a wrapped <code>NonNull</code> pointer.\nConsumes the <code>Box</code>, returning a wrapped <code>NonNull</code> pointer and …\nDecomposes a <code>Vec&lt;T&gt;</code> into its raw components: …\nDecomposes a <code>Vec&lt;T&gt;</code> into its raw components: …\nConverts a <code>Box&lt;T&gt;</code> into a <code>Pin&lt;Box&lt;T&gt;&gt;</code>. If <code>T</code> does not …\nConsumes the <code>Box</code>, returning a wrapped raw pointer.\nDecomposes a <code>Vec&lt;T&gt;</code> into its raw components: …\nDecomposes a <code>String</code> into its raw components: …\nDecomposes a <code>Vec&lt;T&gt;</code> into its raw components: …\nConsumes the <code>Box</code>, returning a wrapped raw pointer and the …\nReturns <code>true</code> if the deque is empty.\nReturns <code>true</code> if the vector contains no elements.\nReturns <code>true</code> if this <code>String</code> has a length of zero, and <code>false</code>…\nReturns a front-to-back iterator.\nReturns a front-to-back iterator that returns mutable …\nConsumes and leaks the <code>Box</code>, returning a mutable reference, …\nConsumes and leaks the <code>Vec</code>, returning a mutable reference …\nConsumes and leaks the <code>String</code>, returning a mutable …\nReturns the number of elements in the deque.\nReturns the number of elements in the vector, also …\nReturns the length of this <code>String</code>, in bytes, not <code>char</code>s or …\nRearranges the internal storage of this deque so it is one …\nAllocates memory on the heap and then places <code>x</code> into it.\nCreates an empty deque.\nConstructs a new, empty <code>Vec&lt;T&gt;</code>.\nCreates a new empty <code>String</code>.\nAllocates memory in the given allocator then places <code>x</code> into …\nCreates an empty deque.\nConstructs a new, empty <code>Vec&lt;T, A&gt;</code>.\nConstructs a new box with uninitialized contents.\nConstructs a new box with uninitialized contents in the …\nConstructs a new boxed slice with uninitialized contents.\nConstructs a new boxed slice with uninitialized contents …\nConstructs a new <code>Box</code> with uninitialized contents, with the …\nConstructs a new <code>Box</code> with uninitialized contents, with the …\nConstructs a new boxed slice with uninitialized contents, …\nConstructs a new boxed slice with uninitialized contents …\nReturns the index of the partition point according to the …\nConstructs a new <code>Pin&lt;Box&lt;T&gt;&gt;</code>. If <code>T</code> does not implement <code>Unpin</code>…\nConstructs a new <code>Pin&lt;Box&lt;T, A&gt;&gt;</code>. If <code>T</code> does not implement …\nRemoves the last element from a vector and returns it, or …\nRemoves the last character from the string buffer and …\nRemoves the last element from the deque and returns it, or …\nRemoves the first element and returns it, or <code>None</code> if the …\nRemoves and returns the last element in a vector if the …\nAppends an element to the back of a collection.\nAppends the given <code>char</code> to the end of this <code>String</code>.\nAppends an element to the back of the deque.\nPrepends an element to the deque.\nAppends a given string slice onto the end of this <code>String</code>.\nAppends an element if there is sufficient spare capacity, …\nCreates an iterator that covers the specified range in the …\nCreates an iterator that covers the specified mutable …\nFill <code>buf</code> with the contents of the “front” slice as …\nRemoves and returns the element at <code>index</code> from the deque. …\nRemoves and returns the element at position <code>index</code> within …\nRemoves a <code>char</code> from this <code>String</code> at a byte position and …\nRemove all matches of pattern <code>pat</code> in the <code>String</code>.\nRemoves the specified range in the string, and replaces it …\nReserves capacity for at least <code>additional</code> more elements to …\nReserves capacity for at least <code>additional</code> more elements to …\nReserves capacity for at least <code>additional</code> bytes more than …\nReserves the minimum capacity for at least <code>additional</code> more …\nReserves the minimum capacity for at least <code>additional</code> more …\nReserves the minimum capacity for at least <code>additional</code> …\nModifies the deque in-place so that <code>len()</code> is equal to …\nResizes the <code>Vec</code> in-place so that <code>len</code> is equal to <code>new_len</code>.\nModifies the deque in-place so that <code>len()</code> is equal to …\nResizes the <code>Vec</code> in-place so that <code>len</code> is equal to <code>new_len</code>.\nRetains only the elements specified by the predicate.\nRetains only the elements specified by the predicate.\nRetains only the characters specified by the predicate.\nRetains only the elements specified by the predicate.\nRetains only the elements specified by the predicate, …\nRotates the double-ended queue <code>n</code> places to the left.\nRotates the double-ended queue <code>n</code> places to the right.\nForces the length of the vector to <code>new_len</code>.\nShrinks the capacity of the deque with a lower bound.\nShrinks the capacity of the vector with a lower bound.\nShrinks the capacity of this <code>String</code> with a lower bound.\nShrinks the capacity of the deque as much as possible.\nShrinks the capacity of the vector as much as possible.\nShrinks the capacity of this <code>String</code> to match its length.\nReturns the remaining spare capacity of the vector as a …\nCreates a splicing iterator that replaces the specified …\nReturns vector content as a slice of <code>T</code>, along with the …\nSplits the deque into two at the given index.\nSplits the collection into two at the given index.\nSplits the string into two at the given byte index.\nSwaps elements at indices <code>i</code> and <code>j</code>.\nRemoves an element from the vector and returns it.\nRemoves an element from anywhere in the deque and returns …\nRemoves an element from anywhere in the deque and returns …\nCreates owned data from borrowed data, usually by cloning.\nConverts the given value to a <code>String</code>.\nShortens the deque, keeping the first <code>len</code> elements and …\nShortens the vector, keeping the first <code>len</code> elements and …\nShortens this <code>String</code> to the specified length.\nAttempts to convert a <code>Vec&lt;T&gt;</code> into a <code>Box&lt;[T; N]&gt;</code>.\nAttempts to convert a <code>Box&lt;[T]&gt;</code> into a <code>Box&lt;[T; N]&gt;</code>.\nConverts a <code>CString</code> into a <code>String</code> if it contains valid …\nAllocates memory on the heap then places <code>x</code> into it, …\nAllocates memory in the given allocator then places <code>x</code> into …\nConstructs a new box with uninitialized contents on the …\nConstructs a new box with uninitialized contents in the …\nConstructs a new boxed slice with uninitialized contents. …\nConstructs a new boxed slice with uninitialized contents …\nConstructs a new <code>Box</code> with uninitialized contents, with the …\nConstructs a new <code>Box</code> with uninitialized contents, with the …\nConstructs a new boxed slice with uninitialized contents, …\nConstructs a new boxed slice with uninitialized contents …\nTries to reserve capacity for at least <code>additional</code> more …\nTries to reserve capacity for at least <code>additional</code> more …\nTries to reserve capacity for at least <code>additional</code> bytes …\nTries to reserve the minimum capacity for at least …\nTries to reserve the minimum capacity for at least …\nTries to reserve the minimum capacity for at least …\nCreates an empty deque with space for at least <code>capacity</code> …\nConstructs a new, empty <code>Vec&lt;T&gt;</code> with at least the specified …\nCreates a new empty <code>String</code> with at least the specified …\nConstructs a new, empty <code>Vec&lt;T, A&gt;</code> with at least the …\nA contiguous growable array type with heap-allocated …\nCreates a <code>Vec</code> containing the arguments.\nCreates an empty deque with space for at least <code>capacity</code> …\nConstructs a new, empty <code>Vec&lt;T&gt;</code> with at least the specified …\nCreates a new empty <code>String</code> with at least the specified …\nCreates an empty deque with space for at least <code>capacity</code> …\nConstructs a new, empty <code>Vec&lt;T, A&gt;</code> with at least the …\nWrites the value and converts to <code>Box&lt;T, A&gt;</code>.\nA draining iterator for <code>Vec&lt;T&gt;</code>.\nAn iterator which uses a closure to determine if an …\nAn iterator that moves out of a vector.\nA splicing iterator for <code>Vec</code>.\nA contiguous growable array type, written as <code>Vec&lt;T&gt;</code>, short …\nReturns a reference to the underlying allocator.\nReturns a reference to the underlying allocator.\nReturns a reference to the underlying allocator.\nReturns the remaining items of this iterator as a mutable …\nReturns the remaining items of this iterator as a slice.\nReturns the remaining items of this iterator as a slice.\nCreates an empty <code>vec::IntoIter</code>.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nKeep unyielded elements in the source <code>Vec</code>.")