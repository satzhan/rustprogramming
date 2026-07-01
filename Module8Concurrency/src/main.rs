fn main() {
    println!("Hello, world!");
}

// chatgpt on Leetcode 2812
Your first BFS idea is correct: compute every cell’s distance to the nearest thief.
The second part is where the logic bends wrong. This is not 0-1 BFS.
0-1 BFS is for paths where each edge cost is 0 or 1, and you minimize the sum of costs.
Here, each cell has a “safeness value,” and the path score is:
minimum safeness value seen anywhere on that path
So this is a bottleneck path problem: maximize the weakest cell on
    the route. Like asking: “Which road from start
    to end has the highest worst bridge weight limit?” You are not adding anything.
A clean no-Dijkstra solution is:
Multi-source BFS from all thieves to fill distances.
Binary search the answer.
For a guessed safeness mid, BFS only through cells with distance >= mid


// Gemini on Leetcode 2812
However, looking at this problem logically and breaking down the mechanics 
    of the algorithm, 0-1 BFS will not work for the second phase. 
    Here is a breakdown of why your logic hits a wall, the syntax 
    issues in the current code, and an alternative approach that avoids
    Dijkstra entirely.The Logic: Why 0-1 BFS Fails HereYour insight 
    that "the difference between adjacent cells' safeness factors
    can't be larger than 1" is factually correct. The Manhattan distance 
    changes by exactly $+1$, $0$, or $-1$ when moving to an adjacent cell.
    However, the state we are tracking isn't the difference between cells; 
it is the bottleneck of the entire path from (0,0) to (x,y).
    Imagine your path so far has a bottleneck (minimum safeness) 
    of 10. You step into a new cell that has a safeness of 2.
    Your path's bottleneck drops instantly from 10 to 2. 
    That is a drop of 8, not 1. Because the path bottleneck value 
    can plummet by more than 1 in a single step, you cannot use a 
    simple Deque where you just push to the front or back.You need 
    to evaluate the highest available bottlenecks first, which strictly
requires a Priority Queue (Dijkstra) or a Disjoint Set Union (DSU) sorted by weight.




