/**
 * State Machine for Test Execution Flow
 * 
 * States:
 * - IDLE: No test running
 * - RUNNING: Test is executing
 * - PAUSED: Test execution paused
 * - FINISHED: Test completed (success or failure)
 * 
 * Transitions:
 * - IDLE -> RUNNING: startTest()
 * - RUNNING -> PAUSED: pauseTest()
 * - PAUSED -> RUNNING: resumeTest()
 * - RUNNING -> FINISHED: test completes
 * - FINISHED -> IDLE: reset()
 * - Any -> IDLE: stopTest()
 */

export enum ExecutionState {
  IDLE = 'idle',
  RUNNING = 'running',
  PAUSED = 'paused',
  FINISHED = 'finished',
}

export enum ExecutionEvent {
  START = 'start',
  PAUSE = 'pause',
  RESUME = 'resume',
  STOP = 'stop',
  FINISH = 'finish',
  RESET = 'reset',
}

type StateTransition = {
  from: ExecutionState;
  event: ExecutionEvent;
  to: ExecutionState;
};

const transitions: StateTransition[] = [
  { from: ExecutionState.IDLE, event: ExecutionEvent.START, to: ExecutionState.RUNNING },
  { from: ExecutionState.RUNNING, event: ExecutionEvent.PAUSE, to: ExecutionState.PAUSED },
  { from: ExecutionState.PAUSED, event: ExecutionEvent.RESUME, to: ExecutionState.RUNNING },
  { from: ExecutionState.RUNNING, event: ExecutionEvent.FINISH, to: ExecutionState.FINISHED },
  { from: ExecutionState.RUNNING, event: ExecutionEvent.STOP, to: ExecutionState.IDLE },
  { from: ExecutionState.PAUSED, event: ExecutionEvent.STOP, to: ExecutionState.IDLE },
  { from: ExecutionState.FINISHED, event: ExecutionEvent.RESET, to: ExecutionState.IDLE },
  { from: ExecutionState.FINISHED, event: ExecutionEvent.START, to: ExecutionState.RUNNING },
];

export class ExecutionStateMachine {
  private currentState: ExecutionState = ExecutionState.IDLE;

  getState(): ExecutionState {
    return this.currentState;
  }

  canTransition(event: ExecutionEvent): boolean {
    return transitions.some(
      t => t.from === this.currentState && t.event === event
    );
  }

  transition(event: ExecutionEvent): boolean {
    const transition = transitions.find(
      t => t.from === this.currentState && t.event === event
    );

    if (!transition) {
      console.warn(
        `[StateMachine] Invalid transition: ${this.currentState} -> ${event}`
      );
      return false;
    }

    this.currentState = transition.to;
    return true;
  }

  reset(): void {
    this.currentState = ExecutionState.IDLE;
  }
}
