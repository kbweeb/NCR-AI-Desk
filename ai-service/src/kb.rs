//! NCR Tech Solutions knowledge base — contacts, locations, procedures, escalation.

#[derive(Clone, Debug)]
pub struct KbEntry {
    pub id: &'static str,
    pub category: &'static str,
    pub title: &'static str,
    pub body: &'static str,
    pub tags: &'static [&'static str],
}

pub fn entry_by_id(id: &str) -> Option<KbEntry> {
    entries().into_iter().find(|e| e.id == id)
}

pub fn entries() -> Vec<KbEntry> {
    vec![
        KbEntry {
            id: "welcome",
            category: "general",
            title: "What the NCR Tech Solutions AI Desk does",
            body: "I help **NCR Tech Solutions** employees with internal contacts, office locations, \
                   client-delivery procedures, IT access, HR, and compliance. NCR teams build and \
                   support technology for retail, banking, and hospitality — ask about phones, \
                   filing reports, project escalations, PTO, and more.",
            tags: &["help", "what", "can", "you", "do", "hello", "hi", "start", "ncr"],
        },
        KbEntry {
            id: "phone-main",
            category: "contacts",
            title: "NCR Tech Solutions main reception",
            body: "Corporate reception (Atlanta HQ): **+1 (678) 399-3100**. \
                   Hours: Mon–Fri 7:30 AM–6:00 PM ET. After hours, dial **+1 (678) 399-3199** \
                   for the operations coordinator.",
            tags: &[
                "main", "reception", "front", "desk", "telephone", "phone", "number", "ncr",
                "headquarters", "atlanta", "office",
            ],
        },
        KbEntry {
            id: "phone-business",
            category: "contacts",
            title: "Business Solutions department",
            body: "**Business Solutions** (retail & banking accounts): **+1 (678) 399-3220**. \
                   Email: businesssolutions@ncrtechsolutions.com. \
                   For RFP and contract questions, ask for the portfolio manager on duty.",
            tags: &[
                "business",
                "solutions",
                "sales",
                "commercial",
                "department",
                "telephone",
                "phone",
                "number",
                "contact",
                "client",
            ],
        },
        KbEntry {
            id: "phone-engineering",
            category: "contacts",
            title: "Engineering & Product Development",
            body: "**Engineering / Product Development**: **+1 (937) 445-5100** (Dayton tech center) \
                   or **+1 (678) 399-3400** (Atlanta). Email: engineering@ncrtechsolutions.com. \
                   For production incidents, use the **Eng Oncall** bridge in Teams first.",
            tags: &[
                "engineering",
                "product",
                "development",
                "r&d",
                "software",
                "hardware",
                "telephone",
                "phone",
                "technical",
            ],
        },
        KbEntry {
            id: "phone-it",
            category: "contacts",
            title: "NCR IT Service Desk",
            body: "IT Service Desk: **+1 (800) 225-5627** (say \"NCR Tech\"). \
                   Email: helpdesk@ncrtechsolutions.com. Portal: https://helpdesk.ncrtechsolutions.com. \
                   For locked accounts or laptop issues, have your **NCR employee ID** ready.",
            tags: &[
                "it",
                "help",
                "desk",
                "support",
                "telephone",
                "phone",
                "computer",
                "laptop",
                "device",
                "electronic",
                "password",
            ],
        },
        KbEntry {
            id: "phone-hr",
            category: "contacts",
            title: "People Operations (HR)",
            body: "People Operations: **+1 (678) 399-2100**. Email: peopleops@ncrtechsolutions.com. \
                   Walk-in: Atlanta HQ, **Building 3, Floor 2, People Hub**. \
                   Benefits and payroll questions: portal **MyNCR** → People.",
            tags: &["hr", "human", "resources", "people", "payroll", "benefits", "phone", "contact"],
        },
        KbEntry {
            id: "contact-complaints",
            category: "escalation",
            title: "Submit a complaint on your NCR device",
            body: "On your NCR-issued laptop or phone:\n\
                   1. **MyNCR** → **Ethics & Integrity** → **Report a concern**.\n\
                   2. Email **integrity@ncrtechsolutions.com** (encrypted).\n\
                   3. Anonymous hotline: https://integrity.ncrtechsolutions.com.\n\
                   Urgent safety or security: call **+1 (678) 399-9911** (24/7 Security Operations).",
            tags: &[
                "complaint",
                "complaints",
                "ethics",
                "integrity",
                "harassment",
                "grievance",
                "electronic",
                "device",
                "report",
                "send",
                "who",
            ],
        },
        KbEntry {
            id: "client-escalation",
            category: "escalation",
            title: "Escalate a client or delivery issue",
            body: "For active customer deployments:\n\
                   1. Log a **Delivery Incident** in ServiceNow (category: Client Delivery).\n\
                   2. Page the **Regional Delivery Lead** via Teams.\n\
                   3. P1 outages: call **+1 (800) 225-5699** (Client Response Center).\n\
                   Email: delivery-escalation@ncrtechsolutions.com.",
            tags: &[
                "client",
                "customer",
                "escalation",
                "incident",
                "outage",
                "delivery",
                "project",
                "support",
            ],
        },
        KbEntry {
            id: "file-physical-report",
            category: "locations",
            title: "Where to file a physical report (NCR offices)",
            body: "Atlanta HQ: **Records & Compliance**, Building 1, Ground Floor, **Room 1-G14** \
                   (slot labeled \"Internal Reports — NCR Tech\").\n\
                   Dayton tech center: Mail to **Room D-118**, Attn: Records.\n\
                   Use **green routing slip** for operational reports; **gold** for financial. \
                   Questions: records@ncrtechsolutions.com or **+1 (678) 399-3180**.",
            tags: &[
                "file",
                "filing",
                "physical",
                "report",
                "paper",
                "drop",
                "submit",
                "records",
                "office",
                "where",
                "location",
                "atlanta",
                "dayton",
            ],
        },
        KbEntry {
            id: "mailroom",
            category: "locations",
            title: "Mailroom & shipping (Atlanta HQ)",
            body: "Mailroom: Building 2, Basement, **Room 2-B08**. \
                   FedEx/UPS pickup: 4:00 PM ET weekdays. \
                   Internal courier: request in **Facilities** app (category: Inter-office mail).",
            tags: &["mail", "mailroom", "package", "shipping", "courier", "fedex"],
        },
        KbEntry {
            id: "facilities",
            category: "locations",
            title: "Facilities & badge access",
            body: "Facilities: **+1 (678) 399-3300** or facilities@ncrtechsolutions.com. \
                   New badge: Building 1 lobby Security desk (bring photo ID). \
                   Visitor badges: host must pre-register in **Visitor NCR** portal 24h ahead.",
            tags: &["facilities", "badge", "access", "visitor", "security", "building"],
        },
        KbEntry {
            id: "pto-request",
            category: "procedures",
            title: "Request PTO at NCR",
            body: "**MyNCR** → **Time Away** → **Request PTO**. Manager approval required. \
                   Submit planned leave at least 5 business days early. \
                   Balance questions: People Operations **+1 (678) 399-2100**.",
            tags: &["pto", "vacation", "leave", "time", "off", "holiday", "mynacr"],
        },
        KbEntry {
            id: "expense-report",
            category: "procedures",
            title: "Submit travel & expense (T&E)",
            body: "**Concur** (linked from MyNCR) → **Create expense report**. \
                   Attach receipts (PDF). Submit by the **10th** for prior-month travel. \
                   Client-billable expenses: add **project code** from your engagement letter.",
            tags: &["expense", "travel", "concur", "reimbursement", "receipt", "finance"],
        },
        KbEntry {
            id: "password-reset",
            category: "it",
            title: "Reset NCR network password",
            body: "Self-service: https://password.ncrtechsolutions.com (on VPN or corporate Wi‑Fi). \
                   Locked out? IT Service Desk **+1 (800) 225-5627** with your NCR badge number.",
            tags: &["password", "reset", "login", "account", "locked", "mynacr"],
        },
        KbEntry {
            id: "vpn",
            category: "it",
            title: "NCR VPN & remote access",
            body: "Install **NCR Secure Access** from the Software Center (MyNCR → Devices). \
                   Guide: helpdesk.ncrtechsolutions.com → Remote Work. \
                   Issues: helpdesk@ncrtechsolutions.com.",
            tags: &["vpn", "remote", "work", "home", "wifi", "secure", "access"],
        },
        KbEntry {
            id: "retail-support",
            category: "procedures",
            title: "Retail solutions field support",
            body: "For in-store NCR retail deployments: open **Field Support** in ServiceNow \
                   (product: Retail Solutions). Hotline: **+1 (800) 225-5678**. \
                   Include store ID, terminal model, and last error code from the device.",
            tags: &["retail", "store", "pos", "terminal", "field", "solutions"],
        },
        KbEntry {
            id: "banking-support",
            category: "procedures",
            title: "Banking & ATM solutions support",
            body: "Banking platform issues: **ServiceNow** → Banking Solutions → **Incident**. \
                   24/7 bridge: **+1 (800) 225-5688**. \
                   Regulatory or audit requests: compliance@ncrtechsolutions.com.",
            tags: &["banking", "atm", "financial", "branch", "platform"],
        },
    ]
}
