//! Optional, real UI capture using only synthetic demonstration files.
use super::*;

struct Capture<'a> {
    browser: &'a Browser,
    dir: &'a Path,
    started: Instant,
    events: Vec<Value>,
}
impl Capture<'_> {
    fn shot(&mut self, name: &str) -> Result<()> {
        thread::sleep(Duration::from_millis(250));
        let encoded = self.browser.command("GET", "/screenshot", None)?;
        let png = STANDARD.decode(encoded.as_str().ok_or("missing screenshot")?)?;
        fs::write(self.dir.join(format!("{name}.png")), png)?;
        let controls = self.browser.script("return Array.from(document.querySelectorAll('button,input,.xcs-version-node,.xcs-file-preview')).map(e=>{const r=e.getBoundingClientRect();return {text:e.innerText||e.getAttribute('aria-label')||'',x:r.x,y:r.y,width:r.width,height:r.height}}).filter(r=>r.width>0&&r.height>0&&r.y<innerHeight&&r.y+r.height>0)")?;
        self.events.push(json!({"kind":"screenshot","file":format!("{name}.png"),"elapsed_ms":self.started.elapsed().as_millis(),"controls":controls}));
        fs::write(
            self.dir.join("capture.json"),
            serde_json::to_vec_pretty(&self.events)?,
        )?;
        Ok(())
    }
    fn click(&mut self, element: &str, label: &str) -> Result<()> {
        let rect = self
            .browser
            .command("GET", &format!("/element/{element}/rect"), None)?;
        self.events.push(json!({"kind":"click","label":label,"rect":rect,"elapsed_ms":self.started.elapsed().as_millis()}));
        self.browser.click(element)
    }
    fn preview(&mut self, budget: &str, name: &str) -> Result<()> {
        self.click(&self.browser.button("Preview", "预览")?, "预览")?;
        self.browser.css("dialog[open] .xcs-file-preview")?;
        let expected = content(budget);
        wait_for("exact real preview body", || {
            if self
                .browser
                .script("return document.querySelector('.xcs-file-preview')?.textContent")?
                == expected
            {
                Ok(())
            } else {
                Err("preview differs from uploaded body".into())
            }
        })?;
        self.shot(name)?;
        self.click(
            &self.browser.button("Close dialog", "关闭对话框")?,
            "关闭预览",
        )?;
        Ok(())
    }
}
fn content(budget: &str) -> String {
    format!("活动：秋季展览\n预算：{budget}万元\n地点：一号展厅\n")
}
fn nodes(browser: &Browser, count: usize) -> Result<()> {
    wait_for("real history node count", || {
        if browser.script("return document.querySelectorAll('.xcs-version-node').length")? == count
        {
            Ok(())
        } else {
            Err("history has not published the expected versions".into())
        }
    })
}
fn note(browser: &Browser, text: &str) -> Result<()> {
    let edit = browser.css(".xcs-version-node[aria-label='当前版本'] .xcs-version-note-action")?;
    browser.click(&edit)?;
    browser.keys(&browser.css("input[aria-label='编辑备注']")?, text)?;
    browser.keys(&browser.css("input[aria-label='编辑备注']")?, "\u{e007}")?;
    browser.element(
        "xpath",
        &format!("//span[@class='xcs-version-note' and text()='{text}']"),
    )?;
    Ok(())
}

pub(super) fn run(browser: &Browser, temp: &Path, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    browser.cdp(
        "Emulation.setDeviceMetricsOverride",
        json!({"width":1920,"height":1080,"deviceScaleFactor":1,"mobile":false}),
    )?;
    let mut c = Capture {
        browser,
        dir,
        started: Instant::now(),
        events: vec![],
    };
    let input = temp.join("活动方案.txt");
    for (index, budget) in ["8", "10", "12"].iter().enumerate() {
        fs::write(&input, content(budget))?;
        let selector = if index == 0 {
            "input[type='file']"
        } else {
            ".xcs-entry-content input[type='file']"
        };
        browser.keys(
            &browser.css(selector)?,
            input.to_str().ok_or("invalid fixture path")?,
        )?;
        if index == 0 {
            let file = browser.element(
                "xpath",
                "//button[contains(@class,'xcs-file-name')][normalize-space(.)='活动方案.txt']",
            )?;
            c.click(&file, "打开活动方案.txt")?;
        }
        nodes(browser, index + 1)?;
        note(browser, &format!("预算{budget}万元"))?;
        c.shot(&format!("0{}-budget-{budget}-history", index + 1))?;
        c.preview(budget, &format!("0{}-budget-{budget}-preview", index + 1))?;
    }
    // Give the existing shared file branch a readable name through the real editor.
    c.click(
        &browser.button("Rename file branch", "重命名文件分支")?,
        "主分支改名",
    )?;
    let rename = browser.css("input[aria-label='重命名文件分支']")?;
    browser.command("POST", &format!("/element/{rename}/clear"), Some(json!({})))?;
    browser.keys(&rename, "主方案\u{e007}")?;
    browser.element(
        "xpath",
        "//button[@role='combobox' and contains(.,'主方案')]",
    )?;
    c.shot("04-three-versions")?;
    let branch = browser.element("xpath", "//article[contains(@class,'xcs-version-node')][.//span[@class='xcs-version-note' and text()='预算8万元']]//button[normalize-space(.)='创建分支']")?;
    c.click(&branch, "从8万历史节点创建分支")?;
    c.shot("05-branch-name-empty")?;
    browser.keys(&browser.css("input[aria-label='分支名称']")?, "精简方案")?;
    c.shot("06-branch-name-entered")?;
    browser.keys(&browser.css("input[aria-label='分支名称']")?, "\u{e007}")?;
    // Waiting for the UI selector to become enabled ensures mutation publication.
    c.click(
        &browser.css(".xcs-file-branches button[role='combobox']:not([disabled])")?,
        "打开分支选择器",
    )?;
    let option = browser.element("xpath", "//*[@role='option'][contains(.,'精简方案')]")?;
    c.shot("07-branch-created-menu")?;
    c.click(&option, "切换精简方案")?;
    browser.css(".xcs-version-node[aria-label='继承的父版本']")?;
    c.shot("08-branch-selected")?;
    c.preview("8", "09-branch-inherited-8-preview")?;
    fs::write(&input, content("6"))?;
    c.shot("10-before-writing-6")?;
    browser.keys(
        &browser.css(".xcs-entry-content input[type='file']")?,
        input.to_str().ok_or("invalid fixture path")?,
    )?;
    nodes(browser, 4)?;
    note(browser, "精简预算6万元")?;
    c.shot("11-branch-written-6")?;
    c.preview("6", "12-branch-6-preview")?;
    c.click(
        &browser.css(".xcs-file-branches button[role='combobox']:not([disabled])")?,
        "再次打开分支选择器",
    )?;
    c.shot("13-select-main-menu")?;
    c.click(
        &browser.element("xpath", "//*[@role='option'][normalize-space(.)='主方案']")?,
        "切回主方案",
    )?;
    browser.element(
        "xpath",
        "//button[@role='combobox' and contains(.,'主方案')]",
    )?;
    c.preview("12", "14-main-still-12-preview")?;
    c.shot("15-main-and-branch-history")?;
    fs::write(
        dir.join("verification.txt"),
        "Real Chrome / XCS server / production WASM. Synthetic data only.\nVerified byte-exact previews: main 8 -> 10 -> 12; branch inherited 8; branch written 6; main remains 12.\nAll changes made via WebDriver UI controls; no DOM or API response substitution.\n",
    )?;
    println!(
        "PASS demo: real three-version history, branch from 8, inherited preview 8, write 6, main remains 12"
    );
    browser.click(&browser.button("Files", "文件")?)?;
    browser.css("input[type='file']")?;
    Ok(())
}
