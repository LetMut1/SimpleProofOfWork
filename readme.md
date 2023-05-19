This is the very simple TCP-server, that demonstrates the main purpose of the ProofOfWork-ResponseChallenge algorithm. This is not production-ready code, because there is a lot of not handled "bad" moments. So, the focus is only on the ProofOfWork algorithm.
<br>
<br>
The general algorithm is:
1. The client opens a tcp-connection for a Challenge request by sending a Token.
2. The server keeps the Token-Secret match and returns the Secret to complete the Challenge, closing the connection.
3. The client performs a POW, calculating Nonce.
4. The client opens a tcp-connection for a WordOfWisdom request, sending Token and Nonce.
5. The server verifies that the POW has been completed and returns WordOfWisdom, closing the connection.

<br>
For the test task (only) the "classic" hash search algorithm (SHA256(SHA256(Secret + Nonce))) starting from the Nth number of zero bytes is selected.
The constant existence of the probability of hitting a hash with the required number of zero bits is not theoretically proven by me for the current input parameters.
In practice, it turned out to find Nonce for Difficulty::IV every time.
In order to develop a robust POW system based on Nonce lookup, a mathematical function needs to be found,
with the byte distribution parameters necessary for the task, the probability of occurrence of bytes, variance, and similar mathematical parameters,
on the basis of which an algorithm will be selected, for which, in the end, it will be possible to calculate the average parameters that are of interest to the user.