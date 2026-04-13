# HEXA 포팅 블로커 리포트 — airgenome + nexus hooks

> airgenome run.sh (475줄) + nexus hooks (17개 .sh) 전체를 .hexa로 포팅하면서 발견한 언어 제한사항.
> 모두 우회 가능하지만, 네이티브 지원되면 코드가 훨씬 깔끔해짐.

## B1: stderr 출력 불가 (심각도: 높음)

**현상**: `eprintln()` 미지원. `exec("echo ... >&2")` 시 hexa 프로세스의 stderr로 전달되지 않음.
**영향**: Claude Code hook은 exit 2 + stderr로 차단 메시지 전달. stderr 없으면 메시지 누락.
**우회**: `println()` + `exit(2)` — stdout으로 대체 (Claude Code가 수용하긴 함)
**요청**: `eprintln(msg)` 또는 `print_stderr(msg)` 내장 함수 추가

```hexa
// 원하는 코드
eprintln("BLOCKED: forbidden extension")
exit(2)

// 현재 우회
println("BLOCKED: forbidden extension")
exit(2)
```

## B2: signal handler (trap) 없음 (심각도: 중간)

**현상**: SIGINT/SIGTERM/EXIT 시 정리 로직 실행 불가.
**영향**: 데몬형 프로그램 (sampler + menubar) 종료 시 자식 프로세스 정리 불가.
**우회**: `exec("bash -c 'trap ... EXIT INT TERM; wait ...'")` 래핑
**요청**: `on_exit(fn)` 또는 `trap(signal, fn)` 콜백 등록

```hexa
// 원하는 코드
on_exit(fn() {
    exec("kill " + sampler_pid + " " + menubar_pid)
    exec("rm -f " + state_file)
})

// 현재 우회
exec("bash -c 'trap \"kill PID1 PID2; rm -f FILE\" EXIT INT TERM; wait PID1 PID2'")
```

## B3: 멀티라인 문자열 연결 불가 (심각도: 낮음)

**현상**: `"line1\n" + \n "line2\n"` 식으로 줄 끝에 `+`를 두고 다음 줄에서 계속할 수 없음.
**영향**: 긴 문자열 (JSON, JXA 코드 등) 인라인 구성 불편.
**우회**: 별도 파일로 분리 (menubar.js), 또는 한 줄에 연결
**요청**: 줄 끝 `+` 연산자 시 다음 줄 연속 허용, 또는 `"""..."""` 멀티라인 리터럴

```hexa
// 원하는 코드
let json = """
{
  "key": "value",
  "nested": true
}
"""

// 또는
let json = "{\n" +
    "  \"key\": \"value\"\n" +
    "}"

// 현재 우회: 한줄 또는 파일 분리
```

## B4: 세미콜론 미지원 (심각도: 낮음)

**현상**: `stmt1; stmt2` 한 줄에 여러 문장 불가.
**영향**: 간단한 가드절 + continue/return 패턴이 장황해짐.
**우회**: 블록으로 분리

```hexa
// 원하는 코드
if x != "y" { println("skip"); continue }

// 현재 필수
if x != "y" {
    println("skip")
    continue
}
```

## B5: exec() stderr 전파 (심각도: 낮음)

**현상**: `exec("cmd")` 시 subprocess의 stderr가 hexa 프로세스 stderr로 전파되지 않고 소실됨.
**영향**: 에러 디버깅 어려움. subprocess 에러 메시지가 사라짐.
**요청**: `exec_with_stderr(cmd)` 또는 exec 옵션으로 stderr 전파 제어

---

## 포팅 결과 요약

| 원본 | 포팅 | 상태 |
|------|------|------|
| run.sh (475줄) | run.hexa + sampler.hexa + menubar.js | 완동 |
| nexus hooks 17개 .sh | 14개 .hexa (setup 포함) | 완동 |
| airgenome block-forbidden-ext.sh | .hexa | 완동 (B1 우회) |
| anima loop-hook.sh | .hexa | 완동 |

**총 .sh → .hexa 전환: 19개 파일, 블로커 0개 (전부 우회 가능)**
